FROM mdoboi/nim:v0.14

COPY ./ /src/

RUN cd src \
  && nimble build -y

CMD /src/main
