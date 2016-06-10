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

proc broadcastMessage(header : Header, body : string) {.async.} =
  for p in players:
    await p.socket.sendMessage(header, body)

  
proc broadcastListPlayers(ps : seq[Player]) {.async.} =
  let playerList = playerListString(ps)
  await broadcastMessage(newHeader(ListPlayers), playerList)

proc processMessage(header : Header, body : string, player : Player) {.async.} =
  result = successful()
  case header.messageType
  of RequestDuel:
    let ps = players.filter( p => p.status.kind == OnHold and p.id != player.id )
    if player.status.kind != OnHold:
      warn("Already in duel " & $player.id)
    else:
      if ps.len > 0 :
        let
          p = ps[random(ps.len)]
          duel = Duel(player1 : player, player2 : p)
        p.status = PlayerStatus(kind : Duelling, duel : duel)
        player.status = PlayerStatus(kind : Duelling, duel : duel)
        debug("send new game to " & $player.id)
        discard broadcastListPlayers(players.filter(proc (p: Player) : bool = p.status.kind == OnHold))
        result = player.socket.answer(header, newHeader(NewGame))
      else :
        debug("send request failed to " & $player.id)
        result = player.socket.answer(header, newHeader(RequestFailed))
  of Proxy:
    case player.status.kind
    of Duelling:
      let
        duel = player.status.duel
        otherPlayer = duel.getOtherPlayer(player)
      debug("proxying message from " & $player.id & " to " & $otherPlayer.id)
      result = otherPlayer.socket.sendMessage(header, body)
    of OnHold:
      await broadcastMessage(header, body)
  of ExitDuel:
    let
      duel = player.status.duel
      otherPlayer = duel.getOtherPlayer(player)
    discard otherPlayer.socket.sendMessage(header)
    duel.player1.status = newOnHoldStatus()
    duel.player2.status = newOnHoldStatus()
  of Name:
    player.name = body
    info("set name " & body)
    await broadcastListPlayers(players)
  of ListPlayers:
    let playerList = playerListString(players)
    result = player.socket.answer(header, newHeader(ListPlayers), playerList)
  else:
    warn("header not managed " & $header)

proc disconnectPlayer(player : Player) {.async.} =
  debug("Disconnected player " & $player.id)
  if player.status.kind == Duelling:
    await player.status.duel.getOtherPlayer(player).socket.sendMessage(newHeader(ExitDuel))
  for i, p in players:
    if p.id == player.id:
      players.del i
  await broadcastListPlayers(players)

proc processPlayer*(player : Player) {.async.} =
  var running = true
  await player.socket.sendMessage(newHeader(Welcome), "Welcome apprentice")
    
  while running:
    let future = player.socket.recvLine()
    let line = await future
    debug("line " & line)
    if future.error != nil or line == "" :
      running = false
      await disconnectPlayer(player)
    else :
      let
        header = parseHeader(future.read)
        body = if header.messageLength > 0: await player.socket.recv(header.messageLength) else:  ""
      
      debug("receive message with header " & $header)
      let msgFuture = processMessage(header, body, player)
      if msgFuture.error != nil:
        running = false
        await disconnectPlayer(player)
      await msgFuture
  
