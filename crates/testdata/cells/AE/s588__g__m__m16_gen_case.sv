module top;
  function automatic logic [3:0] fd(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
  case (fd(2)) 4'd1: begin : g1 initial $display("C1"); end 4'd2: begin : g2 initial $display("C2"); end default: begin : gd initial $display("CD"); end endcase
  initial begin #2 $display("done"); $finish; end
  initial #100 $finish;
endmodule
