import requests
import random
import os

from .site_data import sites_data_dict
from .pretty_print import *

site = "MixDrop"

# THIS IS JUST BROKEN FOR NO REASON. SAYS INVALID EMAIL FOR NO FUCKING REASON AND IM DONE WITH IT. the code itself works 100% just the email part is fucked for god knows what reason.

class MixDrop:
    
    def Uploader(file, proxy_list, user_agents, config):
        try:
            ua = random.choice(user_agents)
            upload_url = sites_data_dict[site]["url"]
            base_url = sites_data_dict[site]["download_url_base"]
            
            email = config.get("api_keys").get(site.lower()).get("email")
            print(email)
            apiKey = config.get("api_keys").get(site.lower()).get("apiKey")
            
            # Get file name
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
            
            files_data = {'file': (os.path.basename(file), open(str(file), 'rb'), 'multipart/form-data'), "email": email, 'key': apiKey}
            
            
            if proxy_list == []:
                req = requests.post(url=upload_url, files=files_data, headers={"User-Agent": ua}).json()
            else:
                req = requests.post(url=upload_url, files=files_data, headers={"User-Agent": ua}, proxies=random.choice(proxy_list)).json
        
            if req.get("success", "") == True:
                return {"status": "ok", "file_name": file_name, "file_url": base_url + req.get("result", {}).get("fileref", "")}
            else:
                raise Exception(req.get("result", {}).get("msg", ""))
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": req}