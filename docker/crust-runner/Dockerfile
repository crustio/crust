# crust build base image
From "ubuntu:20.04"

RUN apt-get update
RUN apt-get install -y openssl
COPY crust /opt/crust/crust
COPY start_crust.sh /opt/start_crust.sh

CMD /opt/start_crust.sh

