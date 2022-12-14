# docker-swarm-deploy

A tool for automating deployments from a push to GitHub to your Docker Swarm cluster, without giving your CICD tool SSH access.

## Motivation

It's common to want a CI/CD workflow that automatically deploys a new release onto your servers if a successful candidate is built.
There are many nice ways to do this in the Kubernetes ecosystem (eg [Flux](https://fluxcd.io/)) but not so many nice tools for Docker Swarm.
Most tutorials suggest adding your SSH key to your CI tool and having it deploy onto your server. But having your CI/CD tool SSH into your production/dev
systems isn't ideal from a security standpoint, and users who can run `docker` commands effectively have root control of the system.

Enter `docker-swarm-deploy`: a little tool which runs in your Docker Swarm cluster and exposes a HTTP server. GitHub sends a webhook to it once a new Docker image is created,
which triggers a `docker stack deploy` being ran from within the cluster by this tool.

## Setup

Start with the [config.json template](https://github.com/itsaphel/docker-swarm-deploy/blob/master/config.json) and replace it with your services' data.
The config takes a mapping of Docker image name to service information (the service name and name of the Docker swarm file).

Then log into your Swarm manager and run (replacing the environment variable values):
```
docker service create \
  --name docker-swarm-deploy \
  --with-registry-auth \
  --constraint "node.role==manager" \
  --publish=5123:3000 \
  --mount type=bind,src=/var/run/docker.sock,dst=/var/run/docker.sock \
  --mount type=bind,src=/path/to/your/config.json,dst=/app/config.json \
  -e DOCKER_REGISTRY="ghcr.io" \
  -e DOCKER_USERNAME="github-username" \
  -e DOCKER_PASSWORD="PersonalAccessToken" \
  -e GITHUB_WEBHOOK_SECRET="WebhookSecret" \
  -e INFRA_REPO_PATH="/srv/infra/swarm" \
itsaphel/docker-swarm-deploy:latest
```

For the tool to be able to pull the Docker image, you'll need to specify a username and password for the container registry. For GitHub's Container registry, see [Authenticating to the Container registry](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry#authenticating-to-the-container-registry).

`INFRA_REPO_PATH` should point to a directory which contains your services' Swarm stack files. In this infra directory there should be sub-directories (named after the service name) whose contents contain that service's Swarm stack file.

Other optional environment variables: `DOCKER_PATH` (default `/usr/local/bin/docker`)

Once this tool is running on your cluster, setup a webhook at `https://github.com/yourrepo/settings/hooks` with payload URL `http://yourserver:5123/notify_release`, content type `application/json`, and a secret of your choosing that matches the value of your `GITHUB_WEBHOOK_SECRET` env variable. You only need to send the `Packages`/`Registry packages` event.

You'll now deploy a new image as soon as a new image is built in GitHub Container registry. You'll probably want to use GitHub Actions or something to build those images, of course.

## Caveats

This is a pretty simple tool, made originally for my own use. Hence some of the design choices (eg only supporting GitHub Container registry). If this doesn't work for your use case, feel free to file an issue and I'll see if a tweak is possible. Also, PRs appreciated <3