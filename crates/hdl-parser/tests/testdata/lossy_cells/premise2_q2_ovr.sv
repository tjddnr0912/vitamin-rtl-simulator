module sub #(parameter int W = 8) (output logic [W-1:0] o); logic [W-1:0] v = 'x; logic [W-1:0] v2; assign o = 'z; initial v2 = 'x; endmodule
module sub2 #(parameter int W = 8); logic [W-1:0] v = 'z; endmodule
module t; logic [69:0] o1;
  sub #(.W(70)) u(.o(o1));
  sub2 u2();
  defparam u2.W = 70;
  initial begin #1 $display("A v=%h v2=%h o=%h", u.v, u.v2, o1); $display("B u2v=%h", u2.v); end
  initial #100 $finish;
endmodule
