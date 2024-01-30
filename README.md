# IAIRC-with-db
![image](https://github.com/fraugho/IAIRC-with-db/assets/144178952/754c4e15-d5f7-43e4-9cb2-5765407ede94)
# About
This is a messaging site using websockets powered by rust as the communication protocol and surrealdb as the database.
# How To Run
```
surreal start --log trace --user root --pass root --bind 127.0.0.1:8000 memory
cargo run --release
```
