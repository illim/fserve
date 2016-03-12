import asyncnet, asyncdispatch, math, options, strutils, parseutils, future, sequtils, logging
import random
import util, model

var players* {.threadvar.}: seq[Player]

# Create a player
# give a random id
# add it to the player list
proc addPlayer*(socket : AsyncSocket) : Player =
  let player = Player(id : random(high(int)), socket: socket, status: newOnHoldStatus())
  info("add player " & $player.id)
  players.add player
  player

proc processMessage(header : Header, body : string, player : Player) {.async.} =
  result = successful()
  case header.messageType
  of RequestDuel:
    let ps = players.filter( p => p.status.kind == OnHold )
    if player.status.kind != OnHold:
      warn("Already in duel " & $player.id)
    else:
      if ps.len > 0 :
        let
          p = ps[randomInt(ps.len)]
          duel = Duel(player1 : player, player2 : p)
        p.status = PlayerStatus(kind : Duelling, duel : duel)
        player.status = PlayerStatus(kind : Duelling, duel : duel)
        debug("send new game to " & $player.id)
        result = player.socket.sendHeader(newHeader(NewGame))
      else :
        debug("send request failed to " & $player.id)
        result = player.socket.sendHeader(newHeader(RequestFailed))
  of Proxy:
    let
      duel = player.status.duel
      otherPlayer = duel.getOtherPlayer(player)
    debug("proxying message from " & $player.id & " to " & $otherPlayer.id)
    await otherPlayer.socket.sendHeader(header)
    result = otherPlayer.socket.send(body)
  of ExitDuel:
    let duel = player.status.duel
    duel.player1.status = newOnHoldStatus()
    duel.player2.status = newOnHoldStatus()
  else:
    warn("header not managed " & $header)

proc processPlayer*(player : Player) {.async.} =
  var running = true
  let disconnect = proc() =
    debug("Disconnected player " & $player.id)
    for i, p in players:
      if p.id == player.id:
        players.del i
    running = false
    
  while running:
    let future = player.socket.recvLine()
    let line = await future
    debug("line " & line)
    if future.error != nil or line == "" :
      disconnect()
    else :
      let
        header = parseHeader(future.read)
        body = if header.messageLength > 0: await player.socket.recv(header.messageLength) else:  ""
      
      debug("receive message with header " & $header)
      let msgFuture = processMessage(header, body, player)
      if msgFuture.error != nil:
        disconnect()
      await msgFuture
  
