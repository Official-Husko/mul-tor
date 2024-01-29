import requests
import os
import random

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *

site = "MegaUpload"

"""
Last Checked 26/01/2024
"""

"""

"MegaUpload": {
    "apiKey": False,
    "url": "https://api.megaupload.nz/upload",
    "api_url": "https://api.megaupload.nz/",
    "download_url_base": "https://megaupload.nz/",
    "size_limit_human": 20,
    "size_limit_bytes": 21474836480,
    "size_unit": "GB"
},

"""

class MegaUpload:
    
     def Uploader(file, proxy_list, user_agents, api_keys):
        try:
            ua = random.choice(user_agents)
            upload_url = sites_data_dict[site]["url"]
            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'
            
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
            
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)
            
            if calc_size == "OK":
                files_data = {'file': (os.path.basename(file), open(str(file), 'rb'), 'multipart/form-data')}
                
                if proxy_list == []:
                    req = requests.post(url=upload_url, files=files_data, headers={"User-Agent": ua}).json()
                else:
                    req = requests.post(url=upload_url, files=files_data, headers={"User-Agent": ua}, proxies=random.choice(proxy_list)).json()
                return {"status": "ok", "file_name": file_name, "file_url": req.get("data").get("file").get("url").get("short")}
            else:
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req}