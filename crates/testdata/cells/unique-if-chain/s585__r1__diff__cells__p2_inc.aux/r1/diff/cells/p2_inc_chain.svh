  task automatic tk(input logic x, input logic y);
    unique if (x) r = 1;
    else if (y) r = 2;
  endtask
