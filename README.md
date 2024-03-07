# BlackSignal
![image](https://github.com/fraugho/BlackSignal/assets/144178952/071205c7-d049-4401-adb0-435ac7a1ede8)
# About
This is a messaging site using websockets powered by rust as the communication protocol and surrealdb as the database.
# How To Run
```
redis-server --port 6379 --bind 127.0.0.1 --save "" --appendonly no
surreal start --log trace --user root --pass root --bind 127.0.0.1:8000 memory
cargo run --release
copy link in terminal and paste in browser
```
