FROM alpine:latest

WORKDIR vita 

RUN apk add \
        wget\
        tar\
        gzip
RUN wget https://github.com/junnlikestea/vita/releases/download/0.1.5/vita-0.1.5-x86_64-unknown-linux-musl.tar.gz 
RUN tar -xvf vita-0.1.5-x86_64-unknown-linux-musl.tar.gz && \  
           mv vita-0.1.5-x86_64-unknown-linux-musl/vita . &&\ 
           rm -rf vita-0.1.5-x86_64-unknown-linux-musl.tar.gz vita-0.1.5-x86_64-unknown-linux-musl

ENV HOME /

CMD ["./vita -d"]
ENTRYPOINT ["/vita/vita"]
### Debug Module ###

#CMD ["tail", "-f", "/dev/null"]
