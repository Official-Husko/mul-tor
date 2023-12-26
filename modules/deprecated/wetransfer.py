import requests
import os
import random
import string
import hashlib
from time import sleep

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from .logger import Logger

from main import DEBUG

site = "WeTransfer"

class WeTransfer:
    
     def Uploader(file, proxy_list, user_agents, api_keys):
        try:
            ua = random.choice(user_agents)
            upload_url = sites_data_dict[site]["url"]
            base_url = sites_data_dict[site]["download_url_base"]
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '') if len(file_name) > 240 else file_name # Changed from 255 to 15 as an additional safety net.
            file_name_normal = file_name
            file_name = file_name.replace(" ", "_")

            # Announce the upload to the server
            announce_url = sites_data_dict[site]["announce_url"]
            announce_data = {
                "message": "Uploaded using Mul-Tor on Github!",
                "display_name": file_name,
                "ui_language": "en",
                "files": [
                    {
                        "name": file_name,
                        "size": file_size,
                        "item_type": "file"
                    }
                ]
            }
            headers = {"User-Agent": ua, "Content-Type": "application/json", "Accept": "application/json"}
            announce = requests.post(url=announce_url, json=announce_data, headers=headers)

            # Extract all needed data
            announce_json = announce.json()
            file_id = announce_json.get("id", "")
            short_url = announce_json.get("shortened_url", "")
            security_hash = announce_json.get("security_hash", "")
            storm_upload_token = announce_json.get("storm_upload_token", "")

            # Check if everything is okay
            initialize_url = sites_data_dict[site]["initialize_url"]
            initialize_data = {"items": [
                {
                    "path": file_name,
                    "item_type": "file",
                    "blocks": [{"content_length": file_size}]
                }
            ]}
            headers = {"User-Agent": ua, "Content-Type": "application/json", "Accept": "application/json", "Authorization": f"Bearer {storm_upload_token}"}
            preflight = requests.post(url=initialize_url, json=initialize_data, headers=headers)

            # Requests upload server. It also needs an md5 hash

            # Create an MD5 hash object
            md5_hash = hashlib.md5()

            # Open the file in binary mode and read it in chunks
            with open(file, 'rb') as md5_file:
                while True:
                    # Read a chunk of data from the file
                    chunk = md5_file.read(8192)  # You can adjust the chunk size as needed

                    # If the chunk is empty, we've reached the end of the file
                    if not chunk:
                        break

                    # Update the hash object with the chunk of data
                    md5_hash.update(chunk)

            # Get the hexadecimal representation of the MD5 hash
            md5_hex = md5_hash.hexdigest()

            prepare_blocks_url = sites_data_dict[site]["prepare_blocks_url"]
            blocks_data = {"blocks": [
                {
                    "content_length": file_size,
                    "content_md5_hex": f"{str(md5_hex)}"
                }
            ]}
            headers = {"User-Agent": ua, "Content-Type": "application/json", "Accept": "application/json", "Authorization": f"Bearer {storm_upload_token}"}
            blocks = requests.post(url=prepare_blocks_url, json=blocks_data, headers=headers)

            # Extract all needed data again
            blocks_json = blocks.json()
            server_url = blocks_json.get("data", {}).get("blocks", [])[0].get("presigned_put_url", "")
            block_id = blocks_json.get("data", {}).get("blocks", [])[0].get("block_id", "")
            print(block_id)
            server_md5 = blocks_json.get("data", {}).get("blocks", [])[0].get("put_request_headers", {}).get("Content-MD5", "")
            x_uploader = blocks_json.get("data", {}).get("blocks", [])[0].get("put_request_headers", {}).get("X-Uploader", "")

            headers = {
                "Content-Type": "application/octet-stream",
                "User-Agent": ua,
                "X-Uploader": x_uploader,
                "Content-MD5": server_md5
            }


            files = {'file': (os.path.basename(file), open(str(file), 'rb'), 'application/octet-stream')}
            if proxy_list == []:
                req = requests.put(url=server_url, files=files, headers=headers)
            else:
                req = requests.put(url=server_url, files=files, headers=headers, proxies=random.choice(proxy_list))
                

            check_status_url = sites_data_dict[site]["check_status_url"]
            check_data = {"items": [
                {
                    "path": file_name,
                    "item_type": "file",
                    "block_ids": [block_id]
                }
            ]}

            headers = {"User-Agent": ua, "Content-Type": "application/json", "Accept": "application/json", "Authorization": f"Bearer {storm_upload_token}"}
            check = requests.post(url=check_status_url, json=check_data, headers=headers)
            print(check.text)

            check_json = check.json()

            while True:
                if check_json.get("ok", "") == False:
                    print("Rechecking")
                    sleep(15)
                    check = requests.post(url=check_status_url, json=check_data, headers=headers)
                    check_json = check.json()
                    print(check.text)
                else:
                    print("should be good now")
                    break
            
            check_json = check.json()
            final_id = check_json.get("data", {}).get("batch_after_mutation", {}).get("foreign_id", "")
            
            finalize_url = sites_data_dict[site]["finalize_url"].format(file_id=final_id)
            headers = {"User-Agent": ua, "Content-Type": "application/json", "Accept": "application/json"}
            finalize = requests.put(url=initialize_url, json=check_data, headers=headers)
            
            finalize_json = finalize.json()

            file_id = finalize_json.get("id", "")
            state = finalize_json.get("state", "")
            security_hash = finalize_json.get("security_hash", "")

            download_url = sites_data_dict[site]["download_url_base"].format(file_id=final_id, security_hash=security_hash)

            if state == "downloadable":
                return {"status": "ok", "file_name": file_name_normal, "file_url": download_url}
            else:
                raise Exception("Fuck me it failed somehow")
                
        except Exception as e:
            return {"status": "error", "file_name": file_name_normal, "exception": str(e), "extra": req}