import os

def update_init_py():
    main_directory = os.getcwd()  # Get the current working directory
    modules_directory = os.path.join(main_directory, 'modules')
    init_file_path = os.path.join(modules_directory, '__init__.py')

    # Main program files to be placed at the start
    main_program_files = [
        'user_agents',
        'pretty_print',
        'availability_checker',
        'site_data',
        'proxy_scraper',
        'config_manager',
        'logger',
        'auto_update',
        'preset_manager'
    ]

    with open(init_file_path, 'w') as init_file:
        # Write import statements for main program files
        init_file.write("# Here are the main program files\n")
        for filename in main_program_files:
            init_file.write(f"from .{filename} import *\n")

        # Write separator
        init_file.write("\n# Here are all modules for the sites that are supported\n")

        # Write import statements for other files
        for filename in os.listdir(modules_directory):
            if filename.endswith('.py') and filename != '__init__.py' and filename[:-3] not in main_program_files:
                module_name = os.path.splitext(filename)[0]
                init_file.write(f"from .{module_name} import *\n")

update_init_py()
