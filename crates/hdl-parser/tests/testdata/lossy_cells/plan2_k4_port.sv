module sub(input logic [69:0] a = '1, input logic [69:0] b = 'z, input logic [69:0] d, output logic [69:0] o); assign o = d; endmodule
module t; logic [69:0] o1, o2; logic c;
  sub u1(.d(c ? 'z : '1), .o(o1));
  sub u2(.a(), .b(('z)), .d(~'0), .o(o2));
  initial begin c=1; #1 $display("A a=%h b=%h o1=%h", u1.a, u1.b, o1); $display("B a=%h b=%h o2=%h", u2.a, u2.b, o2); u1.o = 'z; end
  initial #100 $finish;
endmodule
