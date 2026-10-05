package pk;
  function automatic void fv(input logic x, input logic y, output logic [1:0] r);
    r = 0;
    unique if (x) r = 1;
    else if (y) r = 2;
  endfunction
  task automatic tk(input logic x, input logic y);
    unique if (x) $display("x");
    else if (y) $display("y");
  endtask
endpackage
module sub(input logic a, input logic b);
  always @(a or b) begin
    unique if (a) $display("sub a t=%0t", $time);
    else if (b) $display("sub b t=%0t", $time);
  end
endmodule
