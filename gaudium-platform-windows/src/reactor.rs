use gaudium_core::event::{ApplicationEvent, Event};
use gaudium_core::platform;
use gaudium_core::reactor::{Reaction, Reactor, ThreadContext};
use gaudium_core::window::WindowHandle;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::mem;
use std::ptr;
use winapi::shared::minwindef;
use winapi::um::{processthreadsapi, winuser};

use crate::Platform;

thread_local! {
    static EVENT_THREAD: RefCell<Option<*mut React>> = RefCell::new(None);
}

enum PollResult {
    Dispatch(*const winuser::MSG),
    Repoll,
    Abort(minwindef::UINT),
}

trait React {
    fn react(&mut self, event: Event<Platform>) -> Reaction;
    fn enqueue(&mut self, event: Event<Platform>);
}

pub struct EventThread<R>
where
    R: Reactor<Platform>,
{
    reactor: R,
    reaction: Reaction,
    context: ThreadContext,
    thread: minwindef::DWORD,
    queue: VecDeque<Event<Platform>>,
}

impl<R> EventThread<R>
where
    R: Reactor<Platform>,
{
    unsafe fn run_and_abort(mut self) -> ! {
        EVENT_THREAD.with(|thread| {
            *thread.borrow_mut() = Some(mem::transmute::<&'_ mut React, *mut React>(&mut self));
        });
        let mut message = mem::uninitialized();
        loop {
            match self.poll(&mut message) {
                PollResult::Dispatch(message) => {
                    winuser::TranslateMessage(message);
                    winuser::DispatchMessageW(message);
                }
                PollResult::Abort(code) => {
                    EVENT_THREAD.with(|thread| {
                        *thread.borrow_mut() = None;
                    });
                    self.abort(); // Drop the reactor and all state.
                    crate::exit_process(code)
                }
                _ => {}
            }
        }
    }

    unsafe fn poll(&mut self, message: winuser::LPMSG) -> PollResult {
        let parse = |abort, message: winuser::LPMSG| {
            if abort {
                PollResult::Abort((*message).wParam as minwindef::UINT)
            }
            else {
                PollResult::Dispatch(message)
            }
        };
        if let Some(event) = self.queue.pop_back() {
            self.react(event);
            PollResult::Repoll
        }
        else {
            match self.reaction {
                Reaction::Ready => {
                    if winuser::PeekMessageW(message, ptr::null_mut(), 0, 0, winuser::PM_REMOVE)
                        == 0
                    {
                        self.react(Event::Application {
                            event: ApplicationEvent::QueueExhausted,
                        });
                        PollResult::Repoll
                    }
                    else {
                        // Detect `WM_QUIT` just as `GetMessageW` does.
                        parse((*message).message == winuser::WM_QUIT, message)
                    }
                }
                _ => parse(
                    winuser::GetMessageW(message, ptr::null_mut(), 0, 0) == 0,
                    message,
                ),
            }
        }
    }

    fn abort(self) {
        let EventThread { reactor, .. } = self;
        reactor.abort();
    }
}

impl<R> React for EventThread<R>
where
    R: Reactor<Platform>,
{
    fn react(&mut self, event: Event<Platform>) -> Reaction {
        self.reaction = self.reactor.react(&self.context, event);
        match self.reaction {
            Reaction::Abort => unsafe {
                winuser::PostQuitMessage(0);
            },
            _ => {}
        }
        self.reaction
    }

    fn enqueue(&mut self, event: Event<Platform>) {
        self.queue.push_front(event);
    }
}

pub struct Entry;

impl platform::EventThread<Platform> for Entry {
    type Sink = WindowHandle<Platform>;
}

impl platform::Abort<Platform> for Entry {
    fn run_and_abort<R>(context: ThreadContext, _: Self::Sink, reactor: R) -> !
    where
        R: Reactor<Platform>,
    {
        unsafe {
            winuser::IsGUIThread(minwindef::TRUE);
            EventThread::<R>::run_and_abort(EventThread {
                reactor,
                reaction: Default::default(),
                context,
                thread: processthreadsapi::GetCurrentThreadId(),
                queue: VecDeque::with_capacity(16),
            })
        }
    }
}

pub unsafe fn react(event: Event<Platform>) -> Result<Reaction, ()> {
    EVENT_THREAD.with(move |thread| {
        if let Some(thread) = *thread.borrow_mut() {
            let thread = mem::transmute::<*mut React, &mut React>(thread);
            Ok(thread.react(event))
        }
        else {
            Err(())
        }
    })
}

pub unsafe fn enqueue<I>(events: I) -> Result<(), ()>
where
    I: IntoIterator<Item = Event<Platform>>,
{
    EVENT_THREAD.with(move |thread| {
        if let Some(thread) = *thread.borrow_mut() {
            let thread = mem::transmute::<*mut React, &mut React>(thread);
            for event in events {
                thread.enqueue(event);
            }
            Ok(())
        }
        else {
            Err(())
        }
    })
}
