FROM mdoboi/nim:v0.14

ENV SUPERVISOR_VERSION=3.3.0

RUN apk update && apk add -u python py-pip \
  && pip install supervisor==$SUPERVISOR_VERSION 

COPY ./ /src/

RUN cd src \
  && nimble build -y

COPY supervisord.conf /etc/

CMD supervisord --nodaemon --configuration /etc/supervisord.conf
