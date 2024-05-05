import requests
import os
import random
import string
import re
from time import sleep

from .site_data import Site_Data_CLSS, sites_data_dict, Hash_Calculator
from .pretty_print import *
from main import DEBUG

site = "Rapidgator"

"""
I have no idea why these errors occur. It works on single uploas but shits the bed on multiple uploads.

[WinError 267] The directory name is invalid: 'FILENAME.zip'
[Errno 2] No such file or directory: 'FILENAME.zip'
"""

class Rapidgator:
    
     def Uploader(file, proxy_list, user_agents, api_keys):
        raw_req = "None :("
        status = "terrible"
        try:
            ua = random.choice(user_agents)
            download_url_base = sites_data_dict[site]["download_url_base"]
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
            
            headers = {"User-Agent": ua}
            proxies = random.choice(proxy_list) if proxy_list else None

            api_email = api_keys.get("email", False)
            api_password = api_keys.get("password", False)

            if api_email in (False, "") or api_password in (False, ""):
                raise Exception("Missing API Credentials?")

            initialize_url = sites_data_dict[site]["initialize_url"]

            params = {
                "login": api_email,
                "password": api_password
            }

            raw_req = requests.get(url=initialize_url, headers={"User-Agent": ua}, proxies=proxies, timeout=300, params=params)

            raw_req = raw_req.json()

            try:
                token = raw_req.get("response", {}).get("token", "No_Token")
                status = raw_req.get("status", "terrible")
                details = raw_req.get("details", "No_Details")
                space_left = raw_req.get("response", {}).get("user", {}).get("storage", {}).get("left", 0)
                max_file_size = raw_req.get("response", {}).get("user", {}).get("upload", {}).get("max_file_size", 0)
            except Exception as e:
                raise Exception("Login Failed? Please check your account details else report this.")

            if file_size <= max_file_size and space_left > file_size:

                md5_hash = Hash_Calculator.cal_hash(file)

                server_url = sites_data_dict[site]["server_url"]

                params = {
                    "name": file_name,
                    "hash": md5_hash.hexdigest(),
                    "size": file_size,
                    "token": token
                }

                raw_req = requests.get(url=server_url, params=params, headers=headers, proxies=proxies, timeout=300)

                raw_req = raw_req.json()

                status = raw_req.get("status", 418)
                details = raw_req.get("details", "No_Details")

                if not status == 200:
                    raise Exception(details)
                
                state = raw_req.get("response", {}).get("upload", {}).get("state", 99)

                if state == 2:
                    download_url = raw_req.get("response", {}).get("upload", {}).get("file", {}).get("url", "No_FileURL")
                    return {"status": "ok", "file_name": file_name, "file_url": download_url}

                upload_id = raw_req.get("response", {}).get("upload", {}).get("upload_id", "No_UploadID")
                upload_url = raw_req.get("response", {}).get("upload", {}).get("url", "No_UploadURL")

                form_data = {
                            'file': (os.path.basename(file), open(str(file), 'rb'), 'application/octet-stream')
                        }

                raw_req = requests.post(url=upload_url, files=form_data, headers=headers, proxies=proxies, timeout=300, stream=True)

                raw_req = raw_req.json()
                status = raw_req.get("status", "terrible")
                details = raw_req.get("details", "No_Details")

                if not status == 200:
                    try:
                        raise Exception(details)
                    except Exception as e:
                        raise Exception("Well the upload somehow failed. Report this. " + str(e))

                finalize_url = sites_data_dict[site]["finalize_url"]

                params = {
                    "upload_id": upload_id,
                    "token": token
                }

                while True:
                    raw_req = requests.get(url=finalize_url, params=params, headers=headers, proxies=proxies, timeout=300)

                    raw_req = raw_req.json()
                    status = raw_req.get("status", "terrible")
                    details = raw_req.get("details", "No_Details")
                    upload_state = raw_req.get("response", {}).get("upload", {}).get("state", "No_UploadState")

                    if upload_state == 2:
                        download_url = raw_req.get("response", {}).get("upload", {}).get("file", {}).get("url", "No_UploadURL")
                        break

                    sleep(5)

                if status == 200:
                    return {"status": "ok", "file_name": file_name, "file_url": download_url}
                else:
                    raise Exception(f"Details: {details}")
            else:
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": f"{status} {raw_req}"}

"""

Fuck you Rapidgator.

"""