import asyncnet, asyncdispatch, math, strutils, parseutils, times, future

type
  PlayerStatusKind* = enum
    OnHold
    Duelling
  PlayerStatus* = ref object
    case kind*: PlayerStatusKind
    of OnHold: time* : Time
    of Duelling: duel* : Duel

  Player* = ref object
    id* : int
    socket* : AsyncSocket
    status* : PlayerStatus
  Duel* = ref object
    player1* : Player
    player2* : Player

  # Protocol model
  MessageType* = enum
    RequestDuel
    RequestFailed
    NewGame
    Proxy
    ExitDuel

  Header* = ref object
    messageType* : MessageType
    messageLength* : int


# header consists of 2 ints:  messageType;length
# todo use option when it will be fixed https://github.com/nim-lang/Nim/issues/3794    
proc parseHeader*(message : string) : Header =
  let parts = message.split(';')
  Header(messageType: MessageType(parseInt(parts[0])),
                  messageLength: parseInt(parts[1]))
  
proc `$`*(header : Header) : string =
  $ord(header.messageType) & ";" & $header.messageLength

proc newHeader*(messageType : MessageType, messageLength : int = 0) : Header =
  Header(messageType : messageType, messageLength : messageLength)

proc newOnHoldStatus*() : PlayerStatus=
  PlayerStatus(kind: OnHold, time : getTime())
  
proc getOtherPlayer*(duel : Duel, player : Player) : Player =
  if duel.player1.id == player.id :
    duel.player2
  else:
    duel.player1

proc sendHeader*(socket : AsyncSocket, header : Header) :Future[void] {.async.}=
  result = socket.send($header & "\n")
