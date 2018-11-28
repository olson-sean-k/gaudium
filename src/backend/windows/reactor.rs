use std::cell::RefCell;
use std::collections::VecDeque;
use std::mem;
use std::process;
use std::ptr;
use winapi::shared::minwindef;
use winapi::um::{processthreadsapi, winuser};

use backend::windows;
use event::*;
use reactor::{Poll, Reactor, ThreadStatic};

thread_local! {
    static EVENT_THREAD: RefCell<Option<*mut React>> = RefCell::new(None);
}

enum PollResult {
    Dispatch(*const winuser::MSG),
    Repoll,
    Abort(minwindef::UINT),
}

trait React {
    fn react(&mut self, event: Event) -> Poll;
    fn enqueue(&mut self, event: Event);
}

pub struct ThreadContext {
    thread: minwindef::DWORD,
    phantom: ThreadStatic, // Do not implement `Send` and `Sync`.
}

impl ThreadContext {
    unsafe fn new_in_thread() -> Result<Self, ()> {
        winuser::IsGUIThread(minwindef::TRUE);

        Ok(ThreadContext {
            thread: processthreadsapi::GetCurrentThreadId(),
            phantom: ThreadStatic::default(),
        })
    }
}

pub struct EventThread<R>
where
    R: Reactor,
{
    reactor: R,
    context: ThreadContext,
    queue: VecDeque<Event>,
    poll: Poll,
}

impl<R> EventThread<R>
where
    R: Reactor,
{
    fn new(reactor: R, context: ThreadContext) -> Self {
        EventThread {
            reactor,
            context,
            queue: VecDeque::with_capacity(16),
            poll: Default::default(),
        }
    }

    pub fn run_with_reactor<S>(reactor: S) -> !
    where
        S: Into<R>,
    {
        let thread = EventThread::new(reactor.into(), unsafe {
            ThreadContext::new_in_thread().expect("")
        });
        unsafe { thread.run() }
    }

    pub fn run_with_reactor_from<F>(f: F) -> !
    where
        F: 'static + FnOnce(&ThreadContext) -> R,
    {
        let context = unsafe { ThreadContext::new_in_thread().expect("") };
        let reactor = f(&context);
        let thread = EventThread::new(reactor, context);
        unsafe { thread.run() }
    }

    unsafe fn run(mut self) -> ! {
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
                    windows::exit_process(code)
                }
                _ => {}
            }
        }
    }

    // TODO: Support `Poll::Timeout`.
    unsafe fn poll(&mut self, message: winuser::LPMSG) -> PollResult {
        let parse = |abort, message: winuser::LPMSG| {
            if abort {
                unsafe { PollResult::Abort((*message).wParam as minwindef::UINT) }
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
            match self.poll {
                Poll::Ready => {
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
    R: Reactor,
{
    fn react(&mut self, event: Event) -> Poll {
        self.poll = self.reactor.react(&self.context, event);
        match self.poll {
            Poll::Abort => unsafe {
                winuser::PostQuitMessage(0);
            },
            _ => {}
        }
        self.poll
    }

    fn enqueue(&mut self, event: Event) {
        self.queue.push_front(event);
    }
}

pub unsafe fn react(event: Event) -> Result<Poll, ()> {
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
    I: IntoIterator<Item = Event>,
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
