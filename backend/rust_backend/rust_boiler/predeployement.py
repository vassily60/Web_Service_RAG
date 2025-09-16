import os
import sys

def rename_bootstrap_files(root_directory):
    directoryBootstrap = os.listdir(root_directory)
    for currDir in directoryBootstrap:
        print(currDir)
        new_filename = currDir + '_bootstrap.zip'
        os.rename(os.path.join(root_directory,currDir,'bootstrap.zip'), os.path.join(root_directory,currDir,new_filename))


# Specify the root directory
root_directory = 'target/lambda/'

current_path = os.path.dirname(os.path.abspath(sys.argv[0]))
print(current_path)

# Call the function to rename the bootstrap.zip files
rename_bootstrap_files(os.path.join(current_path,root_directory))

print('THIS IS THE END, MY ONLY FRIEND, THE END')