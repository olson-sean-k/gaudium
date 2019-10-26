use gaudium_core::event::{ApplicationEvent, Event};
use gaudium_core::platform;
use gaudium_core::reactor::{Reaction, Reactor, ThreadContext};
use gaudium_core::window::WindowHandle;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::mem;
use std::ptr;
use winapi::shared::{minwindef, windef};
use winapi::um::{processthreadsapi, winuser};

use crate::Binding;

thread_local! {
    static EVENT_THREAD: RefCell<Option<*mut dyn React>> = RefCell::new(None);
}

enum ControlFlow {
    Dispatch(*const winuser::MSG),
    Continue,
    Abort(minwindef::UINT),
}

trait React {
    fn react(&mut self, event: Event<Binding>) -> Reaction;
    fn enqueue(&mut self, event: Event<Binding>);
}

trait MessageExt {
    fn empty() -> Self;

    fn to_exit_code(&self) -> minwindef::UINT;
}

impl MessageExt for winuser::MSG {
    fn empty() -> Self {
        winuser::MSG {
            hwnd: ptr::null_mut(),
            message: 0,
            wParam: 0,
            lParam: 0,
            time: 0,
            pt: windef::POINT { x: 0, y: 0 },
        }
    }

    fn to_exit_code(&self) -> minwindef::UINT {
        self.wParam as minwindef::UINT
    }
}

pub struct EventThread<R>
where
    R: Reactor<Binding>,
{
    reactor: R,
    reaction: Reaction,
    context: ThreadContext,
    thread: minwindef::DWORD,
    queue: VecDeque<Event<Binding>>,
}

impl<R> EventThread<R>
where
    R: Reactor<Binding>,
{
    unsafe fn run_and_abort(mut self) -> ! {
        EVENT_THREAD.with(|thread| {
            *thread.borrow_mut() = Some(mem::transmute::<&'_ mut dyn React, *mut dyn React>(
                &mut self,
            ));
        });
        let mut message = MessageExt::empty();
        loop {
            match self.pop(&mut message) {
                ControlFlow::Dispatch(message) => {
                    winuser::TranslateMessage(message);
                    winuser::DispatchMessageW(message);
                }
                ControlFlow::Abort(code) => {
                    self.abort(); // Drop the reactor and all state.
                    EVENT_THREAD.with(|thread| {
                        *thread.borrow_mut() = None;
                    });
                    crate::exit_process(code)
                }
                _ => {}
            }
        }
    }

    unsafe fn run_and_join(self) {
        unimplemented!()
    }

    unsafe fn pop(&mut self, message: *mut winuser::MSG) -> ControlFlow {
        unsafe fn abort_or_dispatch(message: *mut winuser::MSG) -> ControlFlow {
            if (*message).message == winuser::WM_QUIT {
                ControlFlow::Abort((*message).to_exit_code())
            }
            else {
                ControlFlow::Dispatch(message)
            }
        }

        if let Some(event) = self.queue.pop_back() {
            self.react(event);
            ControlFlow::Continue
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
                        ControlFlow::Continue
                    }
                    else {
                        abort_or_dispatch(message)
                    }
                }
                Reaction::Abort | Reaction::Wait => {
                    winuser::GetMessageW(message, ptr::null_mut(), 0, 0);
                    abort_or_dispatch(message)
                }
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
    R: Reactor<Binding>,
{
    fn react(&mut self, event: Event<Binding>) -> Reaction {
        self.reaction = self.reactor.react(&self.context, event);
        match self.reaction {
            Reaction::Abort => unsafe {
                winuser::PostQuitMessage(0);
            },
            _ => {}
        }
        self.reaction
    }

    fn enqueue(&mut self, event: Event<Binding>) {
        self.queue.push_front(event);
    }
}

pub struct Entry;

impl platform::EventThread<Binding> for Entry {
    type Sink = WindowHandle<Binding>;
}

impl platform::Abort<Binding> for Entry {
    fn run_and_abort<R>(context: ThreadContext, _: Self::Sink, reactor: R) -> !
    where
        R: Reactor<Binding>,
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

pub unsafe fn react(event: Event<Binding>) -> Result<Reaction, ()> {
    EVENT_THREAD.with(move |thread| {
        if let Some(thread) = *thread.borrow_mut() {
            let thread = mem::transmute::<*mut dyn React, &mut dyn React>(thread);
            Ok(thread.react(event))
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
            let thread = mem::transmute::<*mut dyn React, &mut dyn React>(thread);
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
