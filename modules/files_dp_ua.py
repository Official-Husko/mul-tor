import requests
import os
import random
import string

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "Files.dp.ua"

class Files_dp_ua:
    
     def Uploader(file, proxy_list, user_agents, api_key):
        req = "which one of you maggots ate the fucking request huh?"
        try:
            ua = random.choice(user_agents)
            upload_url = sites_data_dict[site]["url"]
            download_url_base = sites_data_dict[site]["download_url_base"]
            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'
            
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
            
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)
            
            characters = string.ascii_lowercase + string.digits + string.ascii_uppercase
            file_id = ''.join(random.choice(characters) for i in range(10))
            access_code = ''.join(random.choice(characters) for i in range(7))

            headers = {"User-Agent": ua}
            proxies = random.choice(proxy_list) if proxy_list else None

            if calc_size == "OK":
                data = {
                    "idfile": file_id,
                    "accesscode": access_code
                }
                form_data = {
                    'files[]': (os.path.basename(file), open(str(file), 'rb'), 'application/octet-stream')
                }
                
                raw_req = requests.post(url=upload_url, data=data, files=form_data, headers=headers, proxies=proxies)

                if raw_req.status_code == 200:
                    return {"status": "ok", "file_name": file_name, "file_url": download_url_base + file_id}
                else:
                    raise Exception(f"Status code: {raw_req.status_code}")
            else:
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req.content}

"""

Author: Husko
Date: 06/10/2023

"""