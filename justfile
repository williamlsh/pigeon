DOCKER_IMAGE := "ghcr.io/williamlsh/pigeon"
IMAGE_TAG := "latest"

build:
    @cargo build -r

image:
    @sudo docker build -t {{DOCKER_IMAGE}}:{{IMAGE_TAG}} .

push:
    @sudo docker push {{DOCKER_IMAGE}}:{{IMAGE_TAG}}
