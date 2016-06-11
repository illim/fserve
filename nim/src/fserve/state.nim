import asyncnet, asyncdispatch, math, options, strutils, parseutils, future, sequtils, logging, random
import util, model

var players* {.threadvar.}: seq[Player]
var requests* {.threadvar.} : seq[Request]

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

proc findPlayerOnHold(id : int) : Option[Player] =
  for p in players:
    if p.status.kind == OnHold and p.id == id:
       return some(p)

proc findRequest(id : int): Option[Request] =
  for r in requests:
    if r.srcId == id:
      return some(r)

proc purgeRequest(id : int) =
  for i, r in requests:
    if r.srcId == id or r.destId == id:
      requests.del i

  
proc processMessage(header : Header, body : string, player : Player) {.async.} =
  result = successful()
  case header.messageType
  of RequestDuel:   
    if player.status.kind != OnHold:
      warn("Already in duel " & $player.id)
    else:
      let
        reqIdOption = catchAll do -> int : parseInt(body)
        pOption = reqIdOption.flatMap do (reqId : int) -> Option[Player] : findPlayerOnHold(reqId)
      if pOption.isSome :
        let
          p = pOption.get
          reqOption = findRequest(p.id)
        if reqOption.isSome:
          let
            request = reqOption.get
            master  = random(@[player, p])
            duel    = Duel(player1 : player, player2 : p)
          p.status = PlayerStatus(kind : Duelling, duel : duel)
          player.status = PlayerStatus(kind : Duelling, duel : duel)
          purgeRequest(player.id)
          purgeRequest(p.id)
          debug("send new game to " & $player.id)
          discard broadcastListPlayers(players.filter(proc (p: Player) : bool = p.status.kind == OnHold))
          result = master.socket.sendMessage(newHeader(NewGame))
        else:
          let request = Request(srcId : player.id, destId : reqIdOption.get)
          requests.add request
          result = p.socket.sendMessage(newHeader(RequestDuel), $player.id)
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
  purgeRequest(player.id)
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
  
