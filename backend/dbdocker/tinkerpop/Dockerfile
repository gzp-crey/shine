FROM tinkerpop/gremlin-server:latest

USER root
COPY ./conf /data/conf
RUN chown -R gremlin:gremlin /data
#USER gremlin

#ENTRYPOINT [""]
#CMD ["bash"]
CMD ["/data/conf/gremlin-server.yaml"]