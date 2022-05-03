#!/bin/bash

set -o allexport && source .env && set +o allexport 

if [ "${k8s_provider}" == "eks" ]; then
    echo " ...."
    echo "Updating your kube context locally ...."
    aws eks update-kubeconfig --name ${TF_VAR_eks_cluster_name}
    cd ../ingress/${k8s_provider}
    echo "Deploying fuel-faucet ingress to ${TF_VAR_eks_cluster_name} ...."
    mv fuel-faucet-ingress.yaml fuel-faucet-ingress.template
    envsubst < fuel-faucet-ingress.template > fuel-faucet-ingress.yaml
    rm fuel-faucet-ingress.template
    kubectl apply -f fuel-faucet-ingress.yaml
else
   echo "You have inputted a non-supported kubernetes provider in your .env"
fi
