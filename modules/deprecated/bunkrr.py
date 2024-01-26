import requests
import os
import random
import uuid
import re

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "Bunkrr"

# https://github.com/Official-Husko/mul-tor/discussions/13

"""

"Bunkrr": {
    "apiKey": True,
    "url": "{server}",
    "api_url": "https://app.bunkrr.su/",
    "download_url_base": "https://bunkrr.ru/d/",
    "server_url": "https://app.bunkrr.su/api/node",
    "options_url": "https://app.bunkrr.su/api/check",
    "finalize_url": "{server}/finishchunks"
},

"""


class Bunkrr:
    def Uploader(file, proxy_list, user_agents, api_keys):
        """
        Uploads a file to a specified site using random user agents and proxies.

        Args:
            file (str): The path to the file to be uploaded.
            proxy_list (list): A list of proxy URLs.
            user_agents (list): A list of user agent strings.

        Returns:
            dict: A dictionary containing the status, file name, file URL, and site.

        Raises:
            Exception: If an error occurs during the upload process.
        """
        raw_req = "None :("
        try:
            download_url_base = sites_data_dict[site]["download_url_base"]
            # Select a random user agent
            ua = random.choice(user_agents)

            # Get the file size and name
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)

            # Truncate the file name if it is too long
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name

            # Select a random proxy, if available
            proxies = random.choice(proxy_list) if proxy_list else None

            api_token = api_keys.get("token", False)

            if api_token in (False, ""):
                raise Exception("Missing API Credentials?")

            options_url = sites_data_dict[site]["options_url"]

            headers = {
                "User-Agent": ua,
                "Accept": "application/json",
            }

            raw_req = requests.get(url=options_url, headers=headers, proxies=proxies, timeout=300)
            raw_req = raw_req.json()

            max_file_size = raw_req.get("maxSize", "0B")
            chunk_size = raw_req.get("chunkSize", {}).get("max", "0B")
            file_blacklist = raw_req.get("stripTags", {}).get("blacklistExtensions", [])
            
            if max_file_size == "0B" or chunk_size == "0B":
                raise Exception("Invalid max file size or chunk size")
            
            # TODO: check if either one is 0 and abort

            units_to_calc = [max_file_size, chunk_size]
            units_calculated = []

            for unit in units_to_calc:
                size_str = unit.lower()
                unit_multiplier = {'b': 1, 'kb': 1024, 'mb': 1024 ** 2, 'gb': 1024 ** 3, 'tb': 1024 ** 4}
                match = re.match(r'^(\d+)([a-z]+)$', size_str)
                
                if match:
                    value, unit = match.groups()
                    bytes_size = int(value) * unit_multiplier.get(unit, 1)
                    units_calculated.append(bytes_size)
                else:
                    raise ValueError("Invalid input format")

            max_file_size = units_calculated[0]
            chunk_size = units_calculated[1]

            file_uuid = str(uuid.uuid4())

            _, file_extension = os.path.splitext(file_name)

            if file_size <= max_file_size:
                if file_extension in file_blacklist:
                    raise Exception("File is blacklisted!")

                # Get server URL
                server_url = sites_data_dict[site]["server_url"]

                headers = {
                    "User-Agent": ua,
                    "Accept": "application/json",
                    "token": api_token
                }

                raw_req = requests.get(url=server_url, headers=headers, proxies=proxies, timeout=300)

                try:
                    raw_req = raw_req.json()
                    if not raw_req.get("success", False) == True:
                        raise Exception("Failed to get server URL. Error: Success was somehow false? Report this!")
                    upload_url = raw_req.get("url", "")

                except Exception as e:
                    raise Exception(f"Failed to get server URL. Error: {e}")

                chunk_size = 25000000
                total_chunks = (file_size + chunk_size - 1) // chunk_size
                chunk_index = 0
                dzchunkbyteoffset = 0

                with open(file, 'rb') as file_data:
                    while True:
                        chunk_data = file_data.read(chunk_size)
                        if not chunk_data:
                            break  # Exit the loop if we've reached the end of the file
                        
                        upload_data = {
                            "dzuuid": file_uuid,
                            "dzchunkindex": chunk_index,
                            "dztotalfilesize": file_size,
                            "dzchunksize": chunk_size,
                            "dztotalchunkcount": total_chunks,
                            "dzchunkbyteoffset": dzchunkbyteoffset
                        }

                        # Prepare the json data to add extra data to the upload
                        form_data = {
                                    'files[]': (os.path.basename(file), chunk_data, 'application/octet-stream')
                                }

                        # Send the upload request with the form data, headers, and proxies
                        raw_req = requests.post(url=upload_url, data=upload_data, files=form_data, headers=headers, proxies=proxies, timeout=300)

                        raw_req = raw_req.json()

                        if not raw_req.get("success", False) == True:
                            raise Exception("Upload failed somehow? Please Report this!")

                        chunk_index += 1
                        dzchunkbyteoffset += chunk_size

                # final request to get the data
                upload_data = {
                    "files": [
                        {
                            "uuid": file_uuid,
                            "original": file_name,
                            "type": "application/octet-stream",
                            "albumid": "",
                            "filelength": "",
                            "age": ""
                        }
                    ]
                }

                finalize_url = sites_data_dict[site]["finalize_url"].format(server=upload_url)
                
                raw_req = requests.post(url=finalize_url, data=upload_data, headers=headers, proxies=proxies, timeout=300)

                json_req = raw_req.json()

                if json_req.get("success", False) == True:
                    file_url = json_req["files"][0]["url"]

                # Return successful message with the status, file name, file URL, and site
                return {"status": "ok", "file_name": file_name, "file_url": file_url}
            else:
                # Return size error message
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
        except Exception as e:
            # Return error message
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req}