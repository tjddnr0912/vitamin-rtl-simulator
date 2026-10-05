module sub #(parameter string MODE = "fast") ();
  case (MODE)
    "slow": begin : g_s initial #1 $display("K1 slow %m"); end
    "fast": begin : g_f initial #1 $display("K1 fast %m"); end
    "fastest": begin : g_x initial #1 $display("K1 fastest %m"); end
    default: begin : g_d initial #1 $display("K1 def %m"); end
  endcase
endmodule
module top;
  sub u0();
  sub #(.MODE("slow")) u1();
  sub #(.MODE("fastest")) u2();
  sub #(.MODE("generic")) u3();
endmodule
