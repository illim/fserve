import asyncdispatch

# why had to use result= ??
proc successful*() : Future[void] {.async.}=
  let fut = newFuture[void]("empty")
  complete(fut)
  result=fut

proc successful*[T](value : T) : Future[T] {.async.}=
  result = newFuture[T]("success")
  complete(result, value)
    
