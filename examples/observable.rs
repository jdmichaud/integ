use std::error::Error;
use std::rc::Rc;

struct Observer<T, E> {
    next: Box<dyn Fn(T)>,
    complete: Box<dyn Fn()>,
    error: Box<dyn Fn(E)>,
}

struct FullObserver<T, E> {
    start: Box<dyn Fn(&mut Subscription)>,
    next: Box<dyn Fn(T)>,
    complete: Box<dyn Fn()>,
    error: Box<dyn Fn(E)>,
}

struct Observable<T, E> {
    subscription_fn: Rc<dyn Fn(&FullObserver<T, E>) -> Box<dyn Fn()>>,
}

struct Subscription {
    cleanup: Option<Box<dyn Fn()>>,
    closed: bool,
}

impl Subscription {
    fn new<T, E, F: Fn() + 'static>(
        observer: FullObserver<T, E>,
        subscription_fn: Rc<dyn Fn(&FullObserver<T, E>) -> F>,
    ) -> Subscription {
        let mut subscription = Subscription {
            cleanup: None,
            closed: false,
        };

        (observer.start)(&mut subscription);
        if !subscription.closed {
            let cleanup = Box::new((subscription_fn)(&observer));
            subscription.cleanup = Some(cleanup);
        }

        return subscription;
    }

    fn unsubscribe(&mut self) {
        if let Some(cleanup) = &self.cleanup {
            (cleanup)();
        }
        self.closed = true;
    }
}

impl<T, E> Observable<T, E> {
    fn new<F: Fn() + 'static>(
        subscription_fn: impl (Fn(&FullObserver<T, E>) -> F) + 'static,
    ) -> Self {
        Observable {
            subscription_fn: Rc::new(move |observer| Box::new(subscription_fn(observer))),
        }
    }

    fn subscribe(&self, next: impl Fn(T) + 'static) -> Subscription {
        Subscription::new(
            FullObserver {
                start: Box::new(|_s| {}),
                next: Box::new(next),
                complete: Box::new(|| {}),
                error: Box::new(|_err| {}),
            },
            self.subscription_fn.clone(),
        )
    }

    fn subscribe_observer(&self, observer: Observer<T, E>) -> Subscription {
        Subscription::new(
            FullObserver {
                start: Box::new(|_s| {}),
                next: observer.next,
                complete: observer.complete,
                error: observer.error,
            },
            self.subscription_fn.clone(),
        )
    }

    fn subscribe_full_observer(&self, observer: FullObserver<T, E>) -> Subscription {
        Subscription::new(observer, self.subscription_fn.clone())
    }
}

fn foo() {
    let observable = Observable::<i32, Box<dyn Error>>::new(|observer| {
        (observer.next)(42);
        (observer.next)(666);
        (observer.complete)();
        return || {
            println!("bim!");
        };
    });

    let mut subscription = observable.subscribe(|value| {
        println!("next {}", value);
    });
    subscription.unsubscribe();

    let some_closure = "yo!";
    observable.subscribe_full_observer(FullObserver {
        start: Box::new(|_subscription| println!("start")),
        next: Box::new(|value| println!("next {}", value)),
        complete: Box::new(move || println!("complete {}", some_closure)),
        error: Box::new(|error| eprintln!("error {:?}", error)),
    });
}

fn main() {
    foo()
}
