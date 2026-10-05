module top;
  logic a, b; logic [1:0] y;
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  assign y = f(a, b);
  initial begin
    a = 0; b = 1;
    $fatal(1, "bye");
  end
endmodule
