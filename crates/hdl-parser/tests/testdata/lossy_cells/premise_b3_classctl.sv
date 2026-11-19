module t;
  class C #(parameter logic [7:0] P = 1, parameter logic [7:0] Q = 9'h1FF, parameter logic [7:0] R = 8'b1x01_0000);
    function int wp(); return $bits(P); endfunction
    function logic [11:0] cq(); return {Q, 4'b0000}; endfunction
    function logic [7:0] vr(); return R; endfunction
  endclass
  C c; C #(8'b0000_1z01, 4'd5) d;
  initial begin c = new; d = new; #1 $display("A wp=%0d cq=%h vr=%b dvr=%b dwp=%0d", c.wp(), c.cq(), c.vr(), d.vr(), d.wp()); end
  initial #100 $finish;
endmodule
