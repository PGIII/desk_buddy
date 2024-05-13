# Desk Buddy WIP

A display for your desk that connects to your PC and shows various data.
Such as the weather, CPU load, temps, etc.

## State of this project

This is currently a work in progress.
The folder desk-display contains a standalone project that runs on an ESP32S3 and displays the current weather.

desk-display-bare-metal is the start of the remote control display. 
Eventually a PC will be able to fully control the display and show anything. 
Making the firmware as simple as possible for the bare metal device (and not requiring a network connection).
The goal is for the desktop software to be the main source of change. 

db-link is the library that describes the protocol and handles parsing.
## TODO:

[x] Create protocol used to communicate over USB (and possibly other transport means in the future)
[x] Test connection to db-server app
[ ] Improve data waits on device
[ ] Implement remote display control
