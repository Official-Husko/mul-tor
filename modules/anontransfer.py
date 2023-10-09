import requests
import os
import random
import uuid

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "AnonTransfer"

class AnonTransfer:
     def Uploader(file, proxy_list, user_agents, api_key):
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
        try:
            download_url_base = sites_data_dict[site]["download_url_base"]
            # Select a random user agent
            ua = random.choice(user_agents)

            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'

            # Get the file size and name
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)

            # Truncate the file name if it is too long
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name

            # Calculate the size unit for the site
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)

            # Set the user agent header
            headers = {
                "User-Agent": ua
            }

            # Select a random proxy, if available
            proxies = random.choice(proxy_list) if proxy_list else None

            file_uuid = str(uuid.uuid4())

            if calc_size == "OK":

                chunk_size = 52428800  # 52.4288 MB
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
                                    'file': (os.path.basename(file), chunk_data, 'application/octet-stream')
                                }

                        upload_url = sites_data_dict[site]["url"]

                        # Send the upload request with the form data, headers, and proxies
                        raw_req = requests.post(url=upload_url, data=upload_data, files=form_data, headers=headers, proxies=proxies, timeout=50)

                        chunk_index += 1
                        dzchunkbyteoffset += chunk_size

                json_req = raw_req.json()

                name = json_req.get("name", "")
                dir = json_req.get("dir", "")

                # Return successful message with the status, file name, file URL, and site
                return {"status": "ok", "file_name": file_name, "file_url": f"{download_url_base}{dir}/{name}"}
            else:
                # Return size error message
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
        except Exception as e:
            # Return error message
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req.content}

"""

Author: Husko
Date: 06/10/2023

"""