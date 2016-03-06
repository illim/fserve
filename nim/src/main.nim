import asyncnet, asyncdispatch, math, logging
import fserve/server

var L = newConsoleLogger()
addHandler(L)
setLogFilter(Level.lvlDebug)

asyncCheck serve()
runForever()
