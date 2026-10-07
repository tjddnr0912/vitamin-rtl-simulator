module top;
  logic [3:0] v = 4'b0100;
  a_onehot: assert #0 ($onehot0(v));        // deferred immediate assertion as a module item (IEEE 1800-2017 16.4)
  b_fail:   assert #0 ($onehot0(4'b0110)) else $display("b_fail fired");
  initial #1 begin $display("v=%b", v); $finish; end
endmodule
