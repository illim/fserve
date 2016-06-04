A player connect
his state is OnHold

He receives a welcome message
1;18;0;0
Welcome apprentice

He send his name
2;4;0;0
toto

He send a duel request
3;0;0;0

search in players list another player OnHold
if none is found, send request failed (todo no player found or all busy or waiting one connects...)
4;0;0;0

other messages are proxy messages


Run with

```
nim --threads:on c -r  src/main.nim
```

Run test with

```
nim --threads:on c -r  tests/testsuite.nim
```
