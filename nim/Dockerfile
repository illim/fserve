FROM mdoboi/nim

COPY ./ /src/

RUN cd src \
  && nimble build -y

CMD /src/main
