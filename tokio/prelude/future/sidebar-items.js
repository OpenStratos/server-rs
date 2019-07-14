initSidebarItems({"enum":[["Either","Combines two different futures yielding the same item and error types into a single type."],["ExecuteErrorKind","Kinds of errors that can be returned from the `Execute::spawn` function."],["Loop","The status of a `loop_fn` loop."]],"fn":[["collect","Creates a future which represents a collection of the results of the futures given."],["done","Creates a new \"leaf future\" which will resolve with the given result."],["empty","Creates a future which never resolves, representing a computation that never finishes."],["failed","Creates a \"leaf future\" from an immediate value of a failed computation."],["finished","Creates a \"leaf future\" from an immediate value of a finished and successful computation."],["lazy","Creates a new future which will eventually be the same as the one created by the closure provided."],["loop_fn","Creates a new future implementing a tail-recursive loop."],["poll_fn","Creates a new future wrapping around a function returning `Poll`."],["select_all","Creates a new future which will select over a list of futures."],["select_ok","Creates a new future which will select the first successful future over a list of futures."]],"struct":[["AndThen","Future for the `and_then` combinator, chaining a computation onto the end of another future which completes successfully."],["CatchUnwind","Future for the `catch_unwind` combinator."],["Empty","A future which is never resolved."],["ExecuteError","Errors returned from the `Spawn::spawn` function."],["Failed","A future representing a value that is immediately ready."],["Flatten","Future for the `flatten` combinator, flattening a future-of-a-future to get just the result of the final future."],["FlattenStream","Future for the `flatten_stream` combinator, flattening a future-of-a-stream to get just the result of the final stream as a stream."],["FromErr","Future for the `from_err` combinator, changing the error type of a future."],["Fuse","A future which \"fuses\" a future once it's been resolved."],["Inspect","Do something with the item of a future, passing it on."],["IntoStream","Future that forwards one element from the underlying future (whether it is success of error) and emits EOF after that."],["Join","Future for the `join` combinator, waiting for two futures to complete."],["Join3","Future for the `join3` combinator, waiting for three futures to complete."],["Join4","Future for the `join4` combinator, waiting for four futures to complete."],["Join5","Future for the `join5` combinator, waiting for five futures to complete."],["JoinAll","A future which takes a list of futures and resolves with a vector of the completed values."],["Lazy","A future which defers creation of the actual future until a callback is scheduled."],["LoopFn","A future implementing a tail-recursive loop."],["Map","Future for the `map` combinator, changing the type of a future."],["MapErr","Future for the `map_err` combinator, changing the error type of a future."],["OrElse","Future for the `or_else` combinator, chaining a computation onto the end of a future which fails with an error."],["PollFn","A future which adapts a function returning `Poll`."],["Select","Future for the `select` combinator, waiting for one of two futures to complete."],["Select2","Future for the `select2` combinator, waiting for one of two differently-typed futures to complete."],["SelectAll","Future for the `select_all` combinator, waiting for one of any of a list of futures to complete."],["SelectNext","Future yielded as the second result in a `Select` future."],["SelectOk","Future for the `select_ok` combinator, waiting for one of any of a list of futures to successfully complete. Unlike `select_all`, this future ignores all but the last error, if there are any."],["Shared","A future that is cloneable and can be polled in multiple threads. Use `Future::shared()` method to convert any future into a `Shared` future."],["SharedError","A wrapped error of the original future that is cloneable and implements Deref for ease of use."],["SharedItem","A wrapped item of the original future that is cloneable and implements Deref for ease of use."],["Then","Future for the `then` combinator, chaining computations on the end of another future regardless of its outcome."]],"trait":[["Executor","A trait for types which can spawn fresh futures."],["Future","Trait for types which are a placeholder of a value that may become available at some later point in time."],["FutureFrom","Asynchronous conversion from a type `T`."],["IntoFuture","Class of types which can be converted into a future."]]});