#!/bin/bash

set -o errexit # abort on nonzero exitstatus
set -o nounset # abort on unbound variable

set -o allexport && source .env && set +o allexport 

if [ "${k8s_provider}" == "eks" ]; then
    echo " ...."
    echo "Updating your kube context locally ...."
    aws eks update-kubeconfig --name ${TF_VAR_eks_cluster_name}
    cd ../ingress/${k8s_provider}
    echo "Deploying fuel-faucet ingress to ${TF_VAR_eks_cluster_name} ...."
    mv faucet-ingress.yaml faucet-ingress.template
    envsubst < faucet-ingress.template > faucet-ingress.yaml
    rm faucet-ingress.template
    kubectl apply -f faucet-ingress.yaml
else
   echo "You have inputted a non-supported kubernetes provider in your .env"
fi
