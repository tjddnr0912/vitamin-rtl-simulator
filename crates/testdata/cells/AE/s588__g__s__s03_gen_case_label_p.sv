module top;
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    fx = t;
  endfunction
  case (4'd0) fx(2): begin : g0 initial $display("L0"); end default: begin : gd initial $display("LD"); end endcase
  initial begin #2 $display("done"); $finish; end
  initial #100 $finish;
endmodule
