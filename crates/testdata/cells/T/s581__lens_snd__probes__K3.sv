module sub #(parameter logic [63:0] MODE = "fast") ();
  case (MODE)
    "slow": begin : g_s initial #1 $display("K3 slow %m"); end
    "fast": begin : g_f initial #1 $display("K3 fast %m"); end
    "fastest": begin : g_x initial #1 $display("K3 fastest %m"); end
    default: begin : g_d initial #1 $display("K3 def %m"); end
  endcase
endmodule
module top;
  sub u0();
  sub #(.MODE("slow")) u1();
  sub #(.MODE("fastest")) u2();
  sub #(.MODE("generic")) u3();
endmodule
