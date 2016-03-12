import unittest, asyncnet, asyncdispatch, threadpool, os, logging
import ../src/fserve/server
import ../src/fserve/model

proc initLogger()=
  var L = newConsoleLogger()
  addHandler(L)
  setLogFilter(Level.lvlDebug)

proc startServer() : void {.thread} =
  initLogger()
  asyncCheck serve()
  runForever()

proc startClient(client : AsyncSocket) {.async} =
  debug("Connecting")
  await client.connect("127.0.0.1", Port(12345))
  debug("Connected")

proc foreverClient(client : ptr AsyncSocket) : void {.thread} =
  initLogger()
  client[] = newAsyncSocket()
  asyncCheck startClient(client[])
  runForever()

initLogger()
spawn startServer()

sleep(500)

suite "scenarios":
  test "client connect & leave":
    let client = newAsyncSocket()
    defer:
      client.close()
    asyncCheck startClient(client)

    poll()
    sleep(200)
    sleep(200)

  test "client join":
    var sockptr = cast[ptr AsyncSocket](alloc0(sizeof(AsyncSocket)))
    defer:
      dealloc(sockptr)
      sockptr[].close()      
    spawn foreverClient(sockptr)
    let client = newAsyncSocket()
    defer: client.close()
    asyncCheck startClient(client)
   
    poll()
    asyncCheck client.sendHeader(newHeader(RequestDuel))
    poll()
    
    sleep(1000)



