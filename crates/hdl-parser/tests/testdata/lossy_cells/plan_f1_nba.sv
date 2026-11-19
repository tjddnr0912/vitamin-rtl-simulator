module m #(parameter W=128)(input clk); logic [W-1:0] Q; logic [39:0] R;
  always @(posedge clk) begin Q <= 'x; R <= 'x; end
endmodule
module t; logic clk=0; m u(clk);
  initial begin u.Q=0; u.R=0; #1 clk=1; #1 $display("A Q=%h R=%h", u.Q, u.R); end
  initial #100 $finish;
endmodule
