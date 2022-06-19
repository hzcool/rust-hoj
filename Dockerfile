FROM ubuntu:20.04
COPY ./bin/zipper /usr/bin/
RUN mkdir /src
WORKDIR /src
RUN mkdir tmp
COPY ./bin/rust-hoj .
COPY ./src/sql  ./sql
COPY .env .



