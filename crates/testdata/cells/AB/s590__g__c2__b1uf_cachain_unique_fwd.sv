module top;
  logic a, b; logic [1:0] y, z;
  function logic [1:0] f(input logic x, input logic z);
    return {x, z};
  endfunction
  function logic [1:0] g(input logic [1:0] v);
    g = 0;
    unique case (v) 2'b01: g = 1; 2'b10: g = 2; endcase
  endfunction
  assign y = f(a, b);
  assign z = g(y);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b z=%b", $time, y, z);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
