# This is the core for all the `BinaryOptionsTools-v2` versions in the different languages

## Supported Languages (todo)
* Python [Done]
* Rust [Done]
* JavaScript 
* C / C++ 
* Java 
* Dart 

## Todo
* Clean the code and add more logging info
* Add functions to clean closed trades history
* Add support for testing for multiple different connections, like passing an iterable
* Add error handling in case there is an error parsing some data, to return an error and not keep waiting (It is for the `send_message` function) --> Done
* Add support for pending requests by `time` and by `price`
* Replace the `tokio::sync::oneshot` channels to `async_channel::channel` and id so it works properly
* Create an example folder with examples for `async` and `sync` versions of the library and for each language supported

### General
* Make `WebSocketClient` struct more general and create some traits like:
  * `Connect` --> How to connect to websocket
  * `Processor` --> How to process every `tokio_tungstenite::tungstenite::Message`
  * `Sender` --> Struct Or class that will work be shared between threads
  * `Data` --> All the possible data management

### Pocket Option
* Add support for Signals (No clue how to start)
* Add support for pending trades (Seems easy and will add a lot new features to the api)

### Important
* **Pocket Option** server works on the timezone: `UTC +2`