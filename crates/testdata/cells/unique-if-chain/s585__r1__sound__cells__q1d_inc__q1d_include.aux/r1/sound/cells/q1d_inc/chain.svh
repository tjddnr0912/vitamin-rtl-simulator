  task automatic tc(input logic x, input logic y);
    unique if (x) $display("x");
    else if (y) $display("y");
  endtask
