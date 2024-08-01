# import zlib
# import json
# import random

# def generate_dummy_json():
#     return {
#         "ts": 1706591063,
#         "pId": 127909,
#         "sId": 948869,
#         "dc": "VA2",
#         "gID": 211,
#         "PB": 1,
#         "pfi": 1,
#         "imprCnt": 8,
#         "rqAdtype": 20,
#         "gctry": "ci",
#         "greg": "mt",
#         "uid": 658,
#         "mid": 522,
#         "sURL": "a1646b18-451c-4c76-8576-41b3c9d1a08d",
#         "cmpg": {
#         "imprIdx": 4,
#         "id": 16790,
#         "adszId": 677,
#         "cookied": 1,
#         "rnk": 5,
#         "csim": 0,
#         "css": 1,
#         "cadtype": 0
#         }
#     }

# data = [generate_dummy_json() for _ in range(1)]

# compressed_data = zlib.compress(json.dumps(data).encode('utf-8'))

# with open('data.zlib', 'wb') as f:
#     f.write(compressed_data)

# print("Compressed data written to data.zlib")


import zlib
import json

# Function to read JSON data from a file
def read_json_from_file(file_path):
    with open(file_path, 'r') as file:
        return json.load(file)

# File path for the input JSON data
input_file_path = 'transformed_random_samples.json'  # Replace with the actual path

# Read the data from the JSON file
data = read_json_from_file(input_file_path)

# Print the number of JSON objects
print(f"Number of JSON objects: {len(data)}\n")

# Compress the JSON data
compressed_data = zlib.compress(json.dumps(data).encode('utf-8'))

# Write the compressed data to a file
with open('data.zlib', 'wb') as f:
    f.write(compressed_data)

print("Compressed data written to data.zlib")
