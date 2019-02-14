# pinguin 🐧
I was wandering around old ish issues of Phrack recently, stumbled across an article on ICMP tunneling, and thought it'd be a fun thing to implement in Rust to learn about somewhat lower-level networking/socket stuff.

I'm using [libpnet](https://github.com/libpnet/libpnet) for relatively simple access to the transport layer and implementing most of the fiddly packet building stuff \~by hand\~.

Obviously this has to be run as root, so probably you shouldn't, you know, actually *use* this as anything other than a toy. And also it doesn't quite work as intended yet.

## Usage
```
[user@both]$ cargo install pinguin
[user@proxy]$ sudo pinguin -s -p 80 remotesiteip
[user@client]$ sudo pinguin -c -p 8080 proxyip
```
