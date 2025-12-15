# LSD Thumbnail (& Avatar) Server

## How to use
Send a POST request to 127.0.0.1:6767 with url encoded form data.
<br>
Example:
<br>
`curl -X POST http://127.0.0.1:6767 -H "Content-Type: application/x-www-form-urlencoded" -d "id=42&job_type=2"`
<br>
And badabim bada boom in no time you're getting shiny new PNG data encoded in Base64 containing your render.

## How to setup
To get this up and running no issues, you'll need to connect to a MySQL/MariaDB server, which for that you are gonna need to fill in the .env.example file with your database credentials. Don't worry. I will not scoop them up or steal them :&nbsp;)
<br>
After that, you're gonna need some data. If you didn't get the clue yet, this server is built to run alongside the [LSD](https://github.com/stuxvii/lsd) webserver. Just pick up the sql schema file from there and insert some data.

## Request types
1. Avatar
2. Accessory (OBJ)

## Issues
Lag when window is unfocused
<br>
I have no clue about it, maybe change your window manager?

## Basic inner workings
### Startup
- Assign receiver and renderer threads
- Load body part OBJ files into memory
- Connect to database in receiver thread
- Start listening on port 6767 and wait for requests
### On request received
#### Receiver
- Parse input (return 404 if fail)
- Fetch data from parsed ID like accessories, and bodycolors in the case of Avatar requests
- Pass on data to the renderer thread
#### Renderer
- Determine type
- Determine data to be rendered
- Request a render with that data (and hardcoded camera settings)
- ???
- Get screen data as PNG
- Encode in Base64 and return
#### On render finished
- If not empty, return Base64 to client who sent request.
- RINSE REPEAT!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!