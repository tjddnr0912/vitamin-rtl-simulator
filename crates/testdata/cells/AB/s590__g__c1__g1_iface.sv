interface ifc;
  logic a, b; logic [1:0] y;
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    unique case ({x, z}) 2'b01: f = 1; 2'b10: f = 2; endcase
  endfunction
  assign y = f(a, b);
endinterface
module top;
  ifc i();
  initial begin
    i.a = 0; i.b = 1;
    #1 $display("t=%0t y=%b", $time, i.y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
