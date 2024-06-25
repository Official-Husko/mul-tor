import requests
import os
import random
import string

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "Up2Share"

"""
Last Checked 24/03/2024
"""

"""

"Up2Share": {
    "apiKey": False,
    "url": "https://up2sha.re/upload",
    "api_url": "https://up2sha.re/",
    "download_url_base": "https://up2sha.re/file?f=",
    "size_limit": 1,
    "size_unit": "GB"
},

"""

"""
This file is marked as deprecated due to a 400 error when uploading files after chunk 0.
I can't figure out why it does that

Full Error:

{
    "error": {
        "message": "Something went wrong. Please try again."
    }
}

"""

class Up2Share:
    
     def Uploader(file, proxy_list, user_agents, api_keys):
        req = "which one of you maggots ate the fucking request huh?"
        try:
            ua = random.choice(user_agents)
            upload_url = sites_data_dict[site]["url"]
            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'
            
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
            
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)

            if calc_size == "OK":
                # Define the maximum chunk size (in bytes)
                max_chunk_size = round(75.976 * 1024 * 1024)  # 75.976 MB in bytes

                # Initialize the upload key
                upload_key = None

                # Calculate the expected number of chunks
                expected_chunks = (file_size + max_chunk_size - 1) // max_chunk_size
                print("expected chunks: " + str(expected_chunks))

                # Open the file in binary mode for reading
                with open(file, 'rb') as file:
                    chunk_number = 0
                    
                    while True:
                        # Read a chunk of the file
                        chunk = file.read(max_chunk_size)
                        
                        # If the chunk is empty, we've reached the end of the file
                        if not chunk:
                            break
                        
                        # Create a dictionary with the file content for multipart encoding
                        files = {
                            'file': ('blob', chunk, 'application/octet-stream'),
                            'chunk': (None, str(chunk_number)),
                            'chunks': (None, str(expected_chunks)),  # Include the expected number of chunks
                            'clientFilename': (None, file_name),
                            'filesize': (None, str(file_size))
                        }
                        
                        # If we have an upload key, add it to the form data
                        if upload_key:
                            files['uploadKey'] = (None, upload_key)
                        
                        # Make a POST request to upload the chunk using multipart encoding
                        if proxy_list == []:
                            raw_req = requests.post(url=upload_url, files=files, headers={"User-Agent": ua})
                        else:
                            raw_req = requests.post(url=upload_url, files=files, headers={"User-Agent": ua}, proxies=random.choice(proxy_list))
                        
                        print(raw_req.text)

                        # Handle the response
                        if raw_req.status_code == 308:
                            print(f"Chunk {chunk_number} uploaded successfully")
                            
                            # Extract the uploadKey from the JSON response using the get() method
                            response_data = raw_req.json()
                            upload_key = response_data.get('result', {}).get('uploadKey')
                            print(upload_key)
                        else:
                            print(f"Failed to upload chunk {chunk_number}. Status code: {raw_req.status_code}")
                        
                        chunk_number += 1

                # Close the file when done
                file.close()

                response = raw_req.json()
                file_url = response.get("result", {}).get("public_url", "")

                if raw_req.status_code == 201:
                    return {"status": "ok", "file_name": file_name, "file_url": file_url}
                else:
                    raise Exception(f"Status code: {raw_req.status_code}")
            else:
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req}