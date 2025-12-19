FROM continuumio/miniconda3:25.3.1-1

RUN apt-get update && \
    apt-get install -y build-essential

RUN conda create -n triton-env python=3.10.12 -y

COPY requirements.txt /tmp/requirements.txt

SHELL ["conda", "run", "--no-capture-output", "-n", "triton-env", "/bin/bash", "-c"]

RUN pip install -r /tmp/requirements.txt && \
    conda install conda-pack -y

ENTRYPOINT ["conda", "run", "--no-capture-output", "-n", "triton-env", "conda-pack", "-o", "/output/python_env.tar.gz"]