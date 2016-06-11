import asyncdispatch, options

# why had to use result= ??
proc successful*() : Future[void] {.async.}=
  let fut = newFuture[void]("empty")
  complete(fut)
  result=fut

proc successful*[T](value : T) : Future[T] {.async.}=
  result = newFuture[T]("success")
  complete(result, value)
    

proc catchAll*[T](f: proc : T) : Option[T] =
  try:
    some(f())
  except:
    none(T)

proc flatMap*[A, B](o : Option[A], f: proc (x : A): Option[B]) : Option[B] =
  if o.isSome:
    f(o.get)
  else:
    none(B)
