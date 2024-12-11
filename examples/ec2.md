# Setting up Frostbow on an AWS EC2 instance

### Create for Instance profile

The following code creates an instance profile with full access to S3 and S3Tables. For production usage please use custom policies instead of `AmazonS3FullAccess` and `AmazonS3TablesFullAccess`.

```bash
aws iam create-role --role-name EC2InstanceRole --assume-role-policy-document '{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"Service":"ec2.amazonaws.com"},"Action":"sts:AssumeRole"}]}' 
aws iam attach-role-policy --role-name EC2InstanceRole --policy-arn arn:aws:iam::aws:policy/AmazonS3FullAccess 
aws iam attach-role-policy --role-name EC2InstanceRole --policy-arn arn:aws:iam::aws:policy/AmazonS3TablesFullAccess 
aws iam create-instance-profile --instance-profile-name EC2InstanceProfile 
aws iam add-role-to-instance-profile --instance-profile-name EC2InstanceProfile --role-name EC2InstanceRole
```

### Create EC2 instance

Use the AWS UI to create a EC2 instance using the previously created instance profile. You can find more information [here](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/EC2_GetStarted.html?icmpid=docs_ec2_console). It's recommended to use a linux instance.

### Install frostbow

SSH into your EC2 instance and run the following command to install frostbowi:

```bash
wget -qO- https://github.com/JanKaul/frostbow/releases/download/v0.2.0/frostbow-Linux-gnu-x86_64.tar.gz | tar xvz
```
%
### Create Table Bucket

Create a Table bucket in the AWS console with the name `my-prefix-warehouse`.

### Run frostbow cli

Execute the follwing command to run the Frostbow CLI:

```bash
./frostbow -u arn:aws:s3tables:us-east-1:123456789:bucket/my-prefix-
```