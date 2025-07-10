/*

hello can you make an insainly lightweight and small UI lib, make it as small as possible just make it look like the "Iced"
make it in purely in rust, (iced is written in rust too) 
i mainly want the "messaging" part, so the thing where i actually send user inputs from the "actual user inputs" to make it process-able without a big callback mess

*/


//! micro_iced - A minimal Iced-like message-passing UI framework
//! Focuses only on the messaging system, no rendering or widgets included

pub trait Application {
    type Message: std::fmt::Debug + Clone;
    
    /// Initialize the application state
    fn new() -> Self;
    
    /// Handle incoming messages and update state
    fn update(&mut self, message: Self::Message);
    
    /// Optional: Get the current UI representation (simplified)
    fn view(&self) -> Option<String> {
        None
    }
}

pub struct Runtime<App>
where
    App: Application,
{
    app: App,
    message_queue: Vec<App::Message>,
}

impl<App> Runtime<App>
where
    App: Application,
{
    pub fn new() -> Self {
        Self {
            app: App::new(),
            message_queue: Vec::new(),
        }
    }
    
    /// Push a message to be processed
    pub fn push_message(&mut self, message: App::Message) {
        self.message_queue.push(message);
    }
    
    /// Process all queued messages
    pub fn process_messages(&mut self) {
        for message in self.message_queue.drain(..) {
            println!("Processing message: {:?}", message);
            self.app.update(message);
        }
    }
    
    /// Get current state representation (if view is implemented)
    pub fn current_view(&self) -> Option<String> {
        self.app.view()
    }
}

// Example usage
#[derive(Debug, Clone)]
enum CounterMessage {
    Increment,
    Decrement,
}

struct Counter {
    value: i32,
}

impl Application for Counter {
    type Message = CounterMessage;
    
    fn new() -> Self {
        Counter { value: 0 }
    }
    
    fn update(&mut self, message: CounterMessage) {
        match message {
            CounterMessage::Increment => self.value += 1,
            CounterMessage::Decrement => self.value -= 1,
        }
        println!("Counter updated: {}", self.value);
    }
    
    fn view(&self) -> Option<String> {
        Some(format!("Current value: {}", self.value))
    }
}

fn main() {
    let mut runtime = Runtime::<Counter>::new();
    
    // Simulate user input
    runtime.push_message(CounterMessage::Increment);
    runtime.push_message(CounterMessage::Increment);
    runtime.push_message(CounterMessage::Decrement);
    
    // Process all messages
    runtime.process_messages();
    
    // View current state
    if let Some(view) = runtime.current_view() {
        println!("UI: {}", view);
    }
}