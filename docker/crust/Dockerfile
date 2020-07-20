# Build crust image
From "ubuntu:20.04"

RUN apt-get update
RUN apt-get install -y openssl
COPY crust /opt/crust/crust
COPY run.sh /opt/run.sh

CMD /opt/run.sh