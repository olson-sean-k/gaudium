use gaudium_core::event::{ApplicationEvent, Event, Resumption};
use gaudium_core::platform;
use gaudium_core::reactor::{Poll, Reaction, Reactor, ThreadContext};
use gaudium_core::window::WindowHandle;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::mem;
use std::ptr;
use winapi::shared::minwindef;
use winapi::um::winuser;

use crate::Binding;

use ApplicationEvent::Flushed;
use ApplicationEvent::Resumed;
use Poll::Ready;
use Poll::Wait;
use Poll::WaitUntil;
use Reaction::Abort;
use Reaction::Continue;

thread_local! {
    static EVENT_THREAD: RefCell<Option<*mut dyn React>> = RefCell::new(None);
}

trait React {
    fn react(&mut self, event: Event<Binding>) -> Reaction;
    fn enqueue(&mut self, event: Event<Binding>);
}

pub struct EventThread<R>
where
    R: Reactor<Binding>,
{
    reactor: R,
    reaction: Reaction<Poll>,
    context: ThreadContext,
    queue: VecDeque<Event<Binding>>,
}

impl<R> EventThread<R>
where
    R: Reactor<Binding>,
{
    fn new(context: ThreadContext, reactor: R) -> Self {
        EventThread {
            reactor,
            reaction: Default::default(),
            context,
            queue: VecDeque::with_capacity(16),
        }
    }

    #[allow(clippy::useless_transmute)]
    unsafe fn run(mut self) -> minwindef::UINT {
        EVENT_THREAD.with(|thread| {
            *thread.borrow_mut() =
                Some(mem::transmute::<&mut dyn React, *mut dyn React>(&mut self));
        });
        let message = &mut mem::zeroed();
        'react: loop {
            while winuser::PeekMessageW(message, ptr::null_mut(), 0, 0, winuser::PM_REMOVE) != 0 {
                if (*message).message == winuser::WM_QUIT {
                    break 'react;
                }
                dispatch(message); // May call `react`.
            }
            self.react(Event::Application { event: Flushed });
            while let Some(event) = self.queue.pop_front() {
                self.react(event);
            }
            self.poll();
            match self.reaction {
                Continue(Wait) | Continue(WaitUntil(_)) => {
                    if winuser::GetMessageW(message, ptr::null_mut(), 0, 0) == 0 {
                        break 'react;
                    }
                    dispatch(message); // May call `react`.
                }
                Continue(Ready) => {}
                Abort => break 'react,
            }
            self.react(Event::Application {
                event: Resumed(Resumption::Poll),
            });
        }
        EVENT_THREAD.with(|thread| {
            *thread.borrow_mut() = None;
        });
        self.abort(); // Drop the reactor and all state.
        if (*message).message == winuser::WM_QUIT {
            (*message).wParam as minwindef::UINT
        }
        else {
            0
        }
    }

    fn abort(self) {
        let EventThread { reactor, .. } = self;
        reactor.abort();
    }

    fn poll(&mut self) -> Reaction<Poll> {
        // Only overwrite the reaction if it is not in the `Abort` state.
        let reaction = self.reactor.poll(&self.context);
        if let Continue(_) = self.reaction {
            self.reaction = reaction;
        }
        reaction
    }
}

impl<R> React for EventThread<R>
where
    R: Reactor<Binding>,
{
    fn react(&mut self, event: Event<Binding>) -> Reaction {
        // Only overwrite the reaction if an `Abort` was emitted.
        let reaction = self.reactor.react(&self.context, event);
        if let Abort = reaction {
            self.reaction = Abort;
        }
        reaction
    }

    fn enqueue(&mut self, event: Event<Binding>) {
        self.queue.push_back(event);
    }
}

pub struct Entry;

impl platform::Abort<Binding> for Entry {
    fn run_and_abort<R>(context: ThreadContext, _: WindowHandle<Binding>, reactor: R) -> !
    where
        R: Reactor<Binding>,
    {
        unsafe {
            let code = EventThread::new(context, reactor).run();
            crate::exit_process(code)
        }
    }
}

impl platform::Join<Binding> for Entry {
    fn run_and_join<R>(context: ThreadContext, _: WindowHandle<Binding>, reactor: R)
    where
        R: Reactor<Binding>,
    {
        unsafe {
            EventThread::new(context, reactor).run();
        }
    }
}

pub unsafe fn react(event: Event<Binding>) -> Result<Reaction, ()> {
    EVENT_THREAD.with(move |thread| {
        if let Some(thread) = *thread.borrow_mut() {
            Ok((*thread).react(event))
        }
        else {
            Err(())
        }
    })
}

pub unsafe fn enqueue<I>(events: I) -> Result<(), ()>
where
    I: IntoIterator<Item = Event<Binding>>,
{
    EVENT_THREAD.with(move |thread| {
        if let Some(thread) = *thread.borrow_mut() {
            for event in events {
                (*thread).enqueue(event);
            }
            Ok(())
        }
        else {
            Err(())
        }
    })
}

unsafe fn dispatch(message: *mut winuser::MSG) {
    winuser::TranslateMessage(message);
    winuser::DispatchMessageW(message); // May call `reactor::react`.
}
