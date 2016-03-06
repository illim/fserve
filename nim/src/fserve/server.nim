import asyncnet, asyncdispatch, math, logging
import state

proc serve*() {.async.} =
  let server = newAsyncSocket()
  players = @[]
  let port = 12345
  server.bindAddr(Port(port))
  info("Listening to " & $port)
  server.listen()
  
  while true:
    let socket = await server.accept()
    let player = addPlayer(socket)
    
    asyncCheck processPlayer(player)
