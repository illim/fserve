import unittest, asyncnet, asyncdispatch, threadpool, os, logging
import ../src/fserve/server

proc initLogger()=
  var L = newConsoleLogger()
  addHandler(L)
  setLogFilter(Level.lvlDebug)

proc startServer() : void {.thread} =
  initLogger()
  asyncCheck serve()
  runForever()

proc startClient(client : AsyncSocket) {.async} =
  debug("Connecting ")
  await client.connect("127.0.0.1", Port(12345))
  debug("Connected")

initLogger()  
spawn startServer()

suite "scenarios":
  test "client connect & leave":
    let client = newAsyncSocket()
    asyncCheck startClient(client)
    
    sleep(2000)
    client.close()


