module top;
  logic a; logic [1:0] y1, y2;
  function logic [1:0] f1(input logic x, input logic z);
    $display("f1 t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  function logic [1:0] f2(input logic x, input logic z);
    $display("f2 t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  assign y1 = f1(1'b0, 1'b1);
  assign y2 = f2(a, 1'b1);
  initial begin
    $display("i0 y1=%b y2=%b", y1, y2);
    a = 0;
    #1 $display("t=%0t y1=%b y2=%b", $time, y1, y2);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
