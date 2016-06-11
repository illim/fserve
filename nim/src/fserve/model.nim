import asyncnet, asyncdispatch, math, strutils, parseutils, times, future, base64, sequtils

type
  PlayerStatusKind* = enum
    OnHold
    Duelling
  PlayerStatus* = ref object
    case kind*: PlayerStatusKind
    of OnHold: time* : Time
    of Duelling: duel* : Duel

  Player* = ref object
    id*     : int
    socket* : AsyncSocket
    status* : PlayerStatus
    name*   : string
  Duel* = ref object
    player1* : Player
    player2* : Player

  Request* = ref object
    srcId* : int
    destId* : int

  # Protocol model
  MessageType* = enum
    Welcome
    Name
    RequestDuel
    RequestFailed
    NewGame
    Proxy
    ExitDuel
    ListPlayers

  Header* = ref object
    messageType*   : MessageType
    messageLength* : int
    messageId*     : int
    answerId*      : int

# header consists of 2 ints:  messageType;length;id;answerid
proc parseHeader*(message : string) : Header =
  let parts = message.split(';')
  Header(messageType: MessageType(parseInt(parts[0])),
         messageLength: parseInt(parts[1]),
         messageId : parseInt(parts[2]),
         answerId : parseInt(parts[3]))
  
proc `$`*(header : Header) : string =
  $ord(header.messageType) & ";" & $header.messageLength & ";" & $header.messageId & ";" & $header.answerId

proc playerString(p : Player) : string =
  encode(p.name) & ":" & $ord(p.status.kind) & ":" & $p.id

proc playerListString*(players : seq[Player]) : string =
  players.map(playerString).join(";")

proc newHeader*(messageType : MessageType, messageLength : int = 0, messageId : int = 0, answerId : int = 0) : Header =
  Header(messageType : messageType, messageLength : messageLength, messageId : messageId, answerId : answerId)

proc newOnHoldStatus*() : PlayerStatus=
  PlayerStatus(kind: OnHold, time : getTime())
  
proc getOtherPlayer*(duel : Duel, player : Player) : Player =
  if duel.player1.id == player.id :
    duel.player2
  else:
    duel.player1

proc sendHeader(socket : AsyncSocket, header : Header) :Future[void] {.async.}=
  result = socket.send($header & "\n")

proc sendMessage*(socket : AsyncSocket, header : Header, body : string = "") :Future[void] {.async.}=
  header.messageLength = body.len
  if body == "" :
     result = socket.sendHeader(header)
  else:
     await socket.sendHeader(header)
     result = socket.send(body)

proc answer*(socket : AsyncSocket, srcHeader : Header, header : Header, body : string = "") :Future[void] {.async.}=
  header.answerId = srcHeader.messageId
  result = socket.sendMessage(header, body)
