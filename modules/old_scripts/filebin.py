import requests
import os
import random
import string

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from .logger import Logger

from main import DEBUG

site = "FileBin"

class FileBin:
    
     def Uploader(file, proxy_list, user_agents, api_keys):
        raw_req = "None :("
        try:
            ua = random.choice(user_agents)
            upload_url = sites_data_dict[site]["url"]
            base_url = sites_data_dict[site]["download_url_base"]
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '') if len(file_name) > 240 else file_name # Changed from 255 to 15 as an additional safety net.
            
            characters = string.ascii_lowercase + string.digits
            random_string = ''.join(random.choice(characters) for i in range(16))

            rand_url = f"{base_url}{random_string}/{file_name}"
                
            headers = {"User-Agent": ua, "Content-Type": "application/octet-stream", "accept": "application/json"}
            proxies = random.choice(proxy_list) if proxy_list else None

            with open(file, "rb") as file_upload:
                raw_req = requests.post(url=rand_url, data=file_upload, headers=headers, proxies=proxies, timeout=300, stream=True)
                file_upload.close()
            if raw_req.status_code == 201:
                return {"status": "ok", "file_name": file_name, "file_url": rand_url}
            else:
                raise Exception(raw_req.status_code)
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req}

