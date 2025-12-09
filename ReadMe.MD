You're gonna need an actual proper MariaDB server and preferably an instance of LSDBLOX running for you to be using this.<br>
Either change up the connection url thing in the main.rs or just have your database have the username "usr" from the get go.<br>
Put your password in a file at the root of this directory (the one this readme is in) called `.env` in this format:
<br>
<br>
```
DB_PASSWORD=HiThisIsMyEpicDataBasePasswordHopeNoOneReadsThisOops!
```
<br>
Alright that's done? Is that okay? Double check it. Done? Alright, trust ya. &nbsp;&nbsp;&nbsp;&nbsp;: &nbsp;&nbsp; )
<br>
Soooooo  you can send it requests via HTTP using something like cURL (if ur 31337 h4x0r) or Insomnia (if ur normie) and it will return PNG data encoded in base64! Yay! Just decode it and put it into a file or whatever.
<br>
<br>
Wait what's that? You don't know what to even send to this server? Just send url encoded html form data bro. How could you not have figured that out?
<br>
<br>
What url encoded html form data? Well here's some example data:
<br>
<br>

```
id: 42
job_type: 2
```
<hr>

> What does any of this mean bro??

Alright so the id? That's just an id to look up shi in a data base. ok? Remember that? the database you setup earlier? you did that right? you at least inserted some data in it right? Gosh darn it bro. How are you gonna render shit you don't even have in your server you dumb ass.
<hr>

> For fucks sake. I mean, thanks for explaining the id part)? What is the job_type tho?

Well for the job type there are currently two (2) job types. avatars, and accessories.
So, for the first job type, that is number 1 (one) please understand that this type is used for calling only and only exclusively the job that will go ahead in the program, fetch information from the database related to the avatar. Yes job type 1 will render the avatar dickhead.
And job type 2 renders accessories. Take in mind this only supports OBJs and PNGs, so no STLs or STDs or roblox meshes or any of that bullshit bro.

Are we done here? Can i go sleep? Actually I don't fucking care about your opinion. I made this software. I did all the hard work YOU go read it and look what it does you fucker. Good night.