# Message for representing information about a machine learning model
# Typically used to communicate details about a model over ROS

Header header          # Metadata including timestamp and coordinate frame

string input_shape     # Shape of the input tensor(s) in the format "height,width,channels" or "height,width,depth,channels"
string input_type      # Data type of the input tensor(s) (e.g., "float32", "uint8")

string output_shape    # Shape of the output tensor(s) in the format "height,width,channels" or "height,width,depth,channels"
string output_type     # Data type of the output tensor(s) (e.g., "float32", "uint8")

string[] labels        # Array of strings representing the labels used by the model, empty if no labels available

string model_types     # Model tasks/types (e.g., ["object_detection", "classification"])
string model_format    # Format of the model (e.g., "DeepViewRT", "HailoRT", "RKNN", "TensorRT", "TFLite")
string model_name      # Name of the model (if available), otherwise use filename without extension or path
