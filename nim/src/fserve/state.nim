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
        result = player.socket.send($newHeader(NewGame))
      else :
        debug("send request failed to " & $player.id)
        result = player.socket.send($newHeader(RequestFailed))
  of Proxy:
    let
      duel = player.status.duel
      otherPlayer = duel.getOtherPlayer(player)
    debug("proxying message from " & $player.id & " to " & $otherPlayer.id)
    await otherPlayer.socket.send($header)
    result = otherPlayer.socket.send(body)
  of ExitDuel:
    let duel = player.status.duel
    duel.player1.status = newOnHoldStatus()
    duel.player2.status = newOnHoldStatus()
  else:
    warn("header not managed " & $header)

proc processPlayer*(player : Player) {.async.} =
  while true:
    let
      line = await player.socket.recvLine()
      header = parseHeader(line)
      body = if header.messageLength > 0: await player.socket.recv(header.messageLength) else:  ""
    
    debug("receive message with header " & $header)
    await processMessage(header, body, player)
