module top;
  logic a, b; logic [1:0] y, z;
  function logic [1:0] f(input logic x, input logic w);
    $display("f t=%0t x=%b w=%b", $time, x, w);
    return {x, w};
  endfunction
  always_comb begin
    z = 0;
    unique case (y) 2'b01: z = 1; 2'b10: z = 2; endcase
  end
  assign y = f(a, b);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b z=%b", $time, y, z);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
