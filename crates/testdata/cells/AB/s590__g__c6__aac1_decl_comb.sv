module top;
  logic a = 1'b0, b = 1'b1; logic [1:0] y, z;
  function logic [1:0] f(input logic x, input logic w);
    $display("f t=%0t x=%b w=%b", $time, x, w);
    return {x, w};
  endfunction
  assign y = f(a, b);
  always_comb begin
    z = 0;
    unique case (y) 2'b01: z = 1; 2'b10: z = 2; endcase
  end
  initial begin
    #1 $display("t=%0t y=%b z=%b", $time, y, z);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
